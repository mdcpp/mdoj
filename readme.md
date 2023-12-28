# MDOJ

[![wakatime](https://wakatime.com/badge/user/6c7a0447-9414-43ab-a937-9081f3e9fc7d/project/5ca22e8e-119f-4183-a942-bbce042f8705.svg)](https://wakatime.com/badge/user/6c7a0447-9414-43ab-a937-9081f3e9fc7d/project/5ca22e8e-119f-4183-a942-bbce042f8705)
[![judger](https://github.com/mdcpp/mdoj/actions/workflows/judger.yml/badge.svg?branch=master)](https://github.com/mdcpp/mdoj/actions/workflows/judger.yml)
[![backend](https://github.com/mdcpp/mdoj/actions/workflows/backend.yml/badge.svg)](https://github.com/mdcpp/mdoj/actions/workflows/backend.yml)

Performance-oriented contest management system for IOI like contest

> :construction: work in progress, please wait until first release

## Highlights

- :feather:Lightweight: Only 50MB for the binary(~~plugin is very large~~)
- :zap:Lighting fast: Using `Rust`+`Grpc-Web` and correct implementation/algorithm
- :rocket:Easy to use: By using docker compose, you can setup the system in minutes
- :stopwatch:Accurate: Directly use cgroupv2(no docker in judger), Report time deviation to frontend
- :lock:Secure: Using nsjail to sandbox user submitted code

## Features

- :whale:Scalable: By using judger cluster, you can scale the system to any size you want
- :file_cabinet:Extensible: By using plugin system, you can add any language you want
- :telescope: Powerful logging using ``Open-Telemetry``

## Quick Start

Copy ``docker/quickstart`` file to your server and run `docker compose up -d`, then open `https://localhost` in your browser.

> Because we use grpc-web(server-side stream), HTTP2 is required, you can ignore it or place cert and key in `./cert` folder.

login as `admin@admin` and start play arounds.

See [wiki](https://github.com/mdcpp/mdoj/wiki) for more details.

## Full Setup(Docker)

> Please download source code from release

1. Copy ``docker/production`` from source code to your folder
2. generate config for judger by starting the judger once, and edit config
3. generate config for backend by starting the backend once
4. download and extract plugin(language support) of your choice to `./plugins`

If you prefer to use default config, you can skip step 2 and 3.

## Setup for development

1. install following package:

- From system package manager: `protobuf-devel`, `gcc`
- From rustup: `rustup`, `cargo`, `just`
- From their website: `docker`, `docker-compose`

Then start reading documents in subfolder of your interest.

> you may need to run ``just prepare`` in ``judger``, ``backend`` subfolder.
