# MDOJ judger

[![staging-judger-publish](https://github.com/mdcpp/mdoj/actions/workflows/judger.yml/badge.svg?branch=staging)](https://github.com/mdcpp/mdoj/actions/workflows/judger.yml)

From high level, the judger is a grpc server that provide `JudgeService`, which is defined in `/proto/judge.proto`.

The interface should provide ability to
- Judge a submission(compile, run, compare output)
- Stream the output of a submission(compile, run, stream)
  Like leetcode, we should provide experience of watching the output of a submission in real time with setting up development environment.
- Get List of available languages
- Get system utilization

## Design

Developing a GRPC server works in a similar way to developing a function, but remote and stateful.

To implement high level function, we need to implement the following functions:

- `filesystem` module: mount a filesystem in userspace
- `sandbox` module: run a program in a sandbox with resource/permission limitation applied
- `language` module: compile/run a program for a specific programing language

See `dev.md` in each module for more details.
