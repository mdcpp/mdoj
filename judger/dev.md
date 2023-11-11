In this document, we will give brief description about some technical detail for those who want to integrate this project to other system.

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
