#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/wait.h>
#include <spawn.h>

#define MAX_SIZE 1048576

int main()
{
    FILE *source = fopen("/src/src.cpp", "w");

    char *code = malloc(MAX_SIZE * sizeof(char));
    fread(code, sizeof(char), MAX_SIZE, stdin);

    fwrite(code, sizeof(char), strlen(code), source);

    char *args[] = {"g++", "/usr/local/bin/g++", "/src/src.cpp", "-lm", "-o", "/src/src.out"};
    int pid, status;
    if (posix_spawn(&pid, "/usr/local/bin/g++", NULL, NULL, args, NULL))
        if (waitpid(pid, &status, 0) != -1)
            if (status == 0)
            {
                printf("0: compile success");
                return 0;
            }

    printf("4: compile error");
    return 0;
}