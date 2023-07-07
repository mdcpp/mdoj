In this document, we will give brief description about some technical detail for those who want to integrate this project to other system.

## Platform difference

Whatever platform your application running on, every byte of memory is equal, but cpu time was not so lucky to be equal.

To make request "quasi equal", use cpu_time_multiplier

## Language difference



## Known Issue

the cgroup won't be automatic delete if application running correctly(did not killed by ``Limiter``)