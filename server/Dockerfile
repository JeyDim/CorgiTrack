# --- Сборка ---
FROM rust:1-slim AS build
WORKDIR /app

# Кэшируем зависимости: сначала только манифесты + пустышки crate.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --bin corgitrack

# Затем реальные исходники. touch обязателен: COPY сохраняет старый mtime файлов
# из контекста, иначе cargo примет их за уже собранные и оставит в образе пустышку.
COPY migrations ./migrations
COPY src ./src
COPY tests ./tests
RUN find src -name '*.rs' -exec touch {} + \
    && cargo build --release --bin corgitrack

# --- Рантайм ---
# distroless/cc несёт glibc + libgcc + CA-сертификаты, без shell и пакетного
# менеджера (меньше поверхность атаки). Тег :nonroot запускает процесс под uid 65532.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=build /app/target/release/corgitrack /usr/local/bin/corgitrack

EXPOSE 8000
ENV BIND_ADDR=0.0.0.0:8000
ENTRYPOINT ["/usr/local/bin/corgitrack"]
