#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <spawn.h>
#include <errno.h>
#include <sys/wait.h>
#define handle(x,e) { \
    if (e == x) \
    { \
        printf("4: %m\n", errno); \
        return 1; \
    } \
}
#define CC "/usr/local/bin/g++"
#define SRC "/src/src.cpp"
#define OUT "/src/src.out"
#define MAX_SIZE 131072

int main()
{
    FILE *source = fopen(SRC, "w");
    handle(source,NULL);

    printf("1: success create file!\n");

    char *code = malloc(MAX_SIZE * sizeof(char));
    size_t len = fread(code, sizeof(char), MAX_SIZE, stdin);

    fwrite(code, sizeof(char), len, source);
    fclose(source);

    char *args[] = {CC, "-x", "c", SRC, "-lm", "-o", OUT, NULL};
    int pid, status;

    handle(chdir("/tmp"),-1);
    handle(execvp(CC, args),-1);
    printf("1: success execv!\n");

    handle(wait(NULL),-1);
    printf("0: success!\n");
    return 1;
}