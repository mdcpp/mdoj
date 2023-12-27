FROM gcc:13
WORKDIR /
COPY compile.c .
COPY execute.sh /
RUN gcc compile.c -o compile
RUN rm compile.c