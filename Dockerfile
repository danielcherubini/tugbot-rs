FROM rust:latest

WORKDIR /usr/src/tugbot
COPY . .

RUN cargo install --path .

CMD ["tugbot"]
