use std::array::from_ref;

use http::{Method, Response as Resp, StatusCode};
use uuid::Uuid;

use crate::{
    domains::Person,
    http::{IntoResponse, Json, Request, Response},
    AppState,
};

use self::payload::NewPerson;

pub async fn route_request(request: Request, app_state: AppState) -> Response {
    macro_rules! routes {
        (
            $($m:ident $p:literal $($v:ident)? => $f:expr),*
            $(, _ => $wc:expr)?
        ) => {
            $(if request.method() == Method::$m {
                let path = request.uri().path()
                    .strip_prefix($p)
                    .map(|s| s.strip_suffix("/").unwrap_or(s));
                if let Some(_path) = path {
                    $(let $v = _path;)?
                    return $f;
                }
            })*
            $(return $wc;)?

        };
    }

    routes!(
        GET "/pessoas/" id => get_person(app_state, id).await,
        GET "/pessoas" => search_people(app_state, request).await,
        POST "/pessoas" => create_person(app_state, request).await,
        GET "/contagem-pessoas" => count_people(app_state).await,
        _ => {
            let msg = format!(
                "Unknown route {} {}",
                request.method(),
                request.uri().path()
            );
            (StatusCode::NOT_FOUND, msg).into_response()
        }
    );
}

async fn get_person(app_state: AppState, id: &str) -> Response {
    let Ok(id): Result<Uuid, _> = id.parse() else {
        return (StatusCode::UNPROCESSABLE_ENTITY, "invalid id").into_response();
    };

    let person = app_state
        .repository
        .find_one(id)
        .await
        .expect("failed to get person");

    (StatusCode::OK, Json(person)).into_response()
}

async fn search_people(app_state: AppState, request: Request) -> Response {
    let Some(term) = request.uri().query().and_then(|q| q.strip_prefix("t=")) else {
        return (StatusCode::BAD_REQUEST, "").into_response();
    };

    let people = app_state
        .repository
        .search_many(term)
        .await
        .expect("failed to search people");

    (StatusCode::OK, Json(people)).into_response()
}

async fn create_person(app_state: AppState, request: Request) -> Response {
    let body = request.into_body().expect("failed to read body");
    let Ok(person): Result<NewPerson , _> = serde_json::from_slice(&body) else {
        return (StatusCode::BAD_REQUEST, "invalid json").into_response();
    };
    let person: Person = person.into();

    if let Err(err) = person.validate() {
        return (StatusCode::UNPROCESSABLE_ENTITY, err.to_string()).into_response();
    }

    let err = app_state.repository.insert_many(from_ref(&person)).await;
    if let Err(err) = err {
        let is_unique_violation = err
            .downcast_ref::<sqlx::Error>()
            .and_then(|err| err.as_database_error())
            .is_some_and(|err| err.is_unique_violation());

        if is_unique_violation {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                "this nickname is already registered",
            )
                .into_response();
        }
    }

    Resp::builder()
        .status(StatusCode::CREATED)
        .header("Location", format!("/pessoas/{}", person.id))
        .body(None)
        .unwrap()
}

async fn count_people(app_state: AppState) -> Response {
    let rows = app_state
        .repository
        .count_people()
        .await
        .expect("failed to count people");

    (StatusCode::OK, rows.to_string()).into_response()
}

mod payload {
    use time::Date;

    use super::*;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub(super) struct NewPerson {
        pub nome: String,
        pub apelido: String,
        pub nascimento: Date,
        pub stack: Option<Vec<String>>,
    }

    impl From<NewPerson> for Person {
        fn from(value: NewPerson) -> Self {
            Self {
                id: Uuid::now_v7(),
                name: value.nome,
                nickname: value.apelido,
                birthday: value.nascimento,
                stack: value.stack,
            }
        }
    }
}
