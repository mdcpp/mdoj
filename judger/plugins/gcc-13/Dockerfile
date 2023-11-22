FROM gcc:13
WORKDIR /
COPY compile.c .
RUN gcc compile.c -o compile
RUN rm compile.c