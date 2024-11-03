ARG RUST_VERSION=1.81.0
ARG APP_NAME=web-app

################################################################################
# Build stage

FROM rust:${RUST_VERSION}-alpine AS build
ARG APP_NAME
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache clang lld musl-dev git openssl-dev libc-dev gcc openssl-libs-static

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=templates,target=templates \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
cargo build --locked --release && \
cp ./target/release/$APP_NAME /bin/server

################################################################################
# Run stage
FROM alpine:3.18 AS final

# Copy the executable from the "build" stage.
COPY --from=build /bin/server /bin/

# Expose the port that the application listens on.
EXPOSE 3000

# What the container should run when it is started.
CMD ["/bin/server"]
