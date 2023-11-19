This document contain two parts, first part is for developer who want to use this project(Developer), second part is for those who want to contribute to this project. 

# Developer

## Features

To understand this project, you need to know following features:

1. http basic auth
2. judging
    - abstracting for language
    - abstracting for platform
3. health check

And then, just follow the setup and communicate with judger with grpc(see `proto/plugin.proto`).

# Contributor

In this chapter, we will give brief description about some technical detail for those who want to integrate this project to other system.

## Outline

accpet ``judger.proto`` without tls, exam basic auth secret if it(``config.secret``) is presented in config.

When a judge request sent, judger then compile the code, execute the binary.

abtractions:

1. container daemon:

container need file system for storage(compiled files), container daemon is responsible for creating dictionary for storage.

2. container:

Provide a nice interface that is similar to ``std::process``

3. limiter:

Monitor resource usage of container, and kill and report it if it exceed limit.

It's reported through ``tokio::sync::oneshot``, which incurs a overhead similar to ``std::sync::Rc``.

4. Nsjail:

basically a wrapper of ``nsjail``, repsonsiible for previllage dropping and namespace setup.
