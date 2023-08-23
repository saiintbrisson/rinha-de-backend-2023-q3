podman stop rinha-postgres rinha-redis || true

podman run --name=rinha-postgres \
    --rm -p 5432:5432 -dti \
    -e POSTGRES_PASSWORD=secret \
    -v ./init.sql:/docker-entrypoint-initdb.d/ddl.sql \
    postgres:15

podman run --name=rinha-redis \
    --rm -p 6379:6379 -dti \
    redis/redis-stack
