# MDOJ

[![wakatime](https://wakatime.com/badge/user/6c7a0447-9414-43ab-a937-9081f3e9fc7d/project/5ca22e8e-119f-4183-a942-bbce042f8705.svg)](https://wakatime.com/badge/user/6c7a0447-9414-43ab-a937-9081f3e9fc7d/project/5ca22e8e-119f-4183-a942-bbce042f8705)

Performance-oriented contest management system for IOI like contest

> Work In Progress

## Highlights

- Lightweight: Only 20MB for the binary(~~plugin is very large~~)
- Lighting fast: Using `Rust`+`Grpc-Web` and correct implementation/algorithm
- Easy to use: By using docker compose, you can setup the system in minutes
- Accurate: Directly use cgroupv2(no docker in judger), Report time deviation to frontend
- Secure: Using nsjail to isolate the program

## Features

- Scalable: By using judger cluster, you can scale the system to any size you want
- Extensible: By using plugin system, you can add any language you want

## Setup

copy `docker/simple/docker-compose.yml`` file to your server and run `docker-compose up -d`

> do not use `docker/docker-compose.yml` file, it is for development only

