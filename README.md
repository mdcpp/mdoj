# MDOJ

[![wakatime](https://wakatime.com/badge/user/6c7a0447-9414-43ab-a937-9081f3e9fc7d/project/5ca22e8e-119f-4183-a942-bbce042f8705.svg)](https://wakatime.com/badge/user/6c7a0447-9414-43ab-a937-9081f3e9fc7d/project/5ca22e8e-119f-4183-a942-bbce042f8705)
[![master](https://github.com/mdcpp/mdoj/actions/workflows/master.yml/badge.svg)](https://github.com/mdcpp/mdoj/actions/workflows/master.yml)
[![staging](https://github.com/mdcpp/mdoj/actions/workflows/staging.yml/badge.svg)](https://github.com/mdcpp/mdoj/actions/workflows/staging.yml)

Performance-oriented contest management system for IOI like contest

> [!IMPORTANT]
> :construction: work in progress, please wait until first release

## Highlights

- :feather:Lightweight: Only 50MB for the binary(~~plugin is very large~~)
- :zap:Lighting fast: Using `Rust`+`Grpc-Web` and correct implementation/algorithm
- :rocket:Easy to use: By using docker compose, you can setup the system in minutes
- :stopwatch:Accurate: Directly use cgroupv2(no docker in judger), Report time deviation to frontend
- :lock:Secure: Using nsjail to sandbox user submitted code

## Features

> [!TIP]
> Because we use grpc-web(server-side stream), HTTP2 is recommended, otherwise users won't be able to see realtime submit update(it's still very usable)

- :whale:Scalable: When deployed in cluster, you can scale the system to satisfy reasonable request.
- :file_cabinet:Extensible: You can add any programing language by placing a `*.lang` file in `plugins` folder
- :telescope: Powerful `metrics`/`tracing` using ``Open-Telemetry``

<details>
  <summary><h2>Quick Start</h2></summary>

   Copy `docker/quickstart` file to your server and run `docker compose up -d`, then open [https://localhost](https://localhost) in your browser.

   login as `admin@admin` and start play arounds.
  
</details>

<details>
  <summary><h2>Full Setup(Docker)</h2></summary>

   1. Copy `docker/production` from source code to your folder
   2. run migration by running `docker compose up migration`
   3. generate config for judger by starting the judger once, and edit config
   4. generate config for backend by starting the backend once
   5. download and extract plugin(language support) of your choice to `./plugins`

   If you prefer to use default config, you can skip step 3 and 4.

   See [wiki](https://github.com/mdcpp/mdoj/wiki) for more details.
  
</details>

<details>
  <summary><h2>Setup for development</h2></summary>

   1. install following package:

   - From system package manager: `protobuf-devel`, `gcc`
   - From rustup: `rustup`, `cargo`, `just`
   - From their website: `docker`, `docker-compose`

   Then start reading documents in subfolder of your interest.

   > you may need to run ``just prepare`` in ``judger``, ``backend`` subfolder.
  
</details>

## Configuration

> [!TIP]
> Set `CONFIG_PATH` to change the path to config file, default value is `config.toml`

See wiki for more detail

## Development

MDOJ contain three service:
1. Frontend: Render first time html, serve wasm.
2. Backend: Serve both frontend and web client(chrome...)
3. judger: run user-submitted code and return resource usage(and output)

> [!TIP]
> See `DEV.md` to understand how to get started. 

See `/backend/README.md`, `judger/README.md`, `frontend/README.md` for more detail.
