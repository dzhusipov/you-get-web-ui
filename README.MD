# You-Get Web UI

This is a web application built with Rust and Actix Web that allows users to download videos using the `you-get` script. The application is containerized using Docker.

## Features

- Download videos from various websites using `you-get`.
- Track download progress.
- Manage downloaded files.

## Prerequisites

- Docker
- Docker Compose (optional)

## Getting Started

### Building the Docker Image

To build the Docker image, run the following command:

```sh
docker build -t you-get-web-ui .