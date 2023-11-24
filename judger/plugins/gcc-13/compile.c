#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <unistd.h>
#include <spawn.h>
#include <errno.h>
#include <sys/wait.h>
#define CC "/usr/local/bin/g++"
#define SRC "/src/src.cpp"
#define OUT "/src/src.out"
#define MAX_SIZE 131072

int main()
{
    FILE *source = fopen(SRC, "w");

    char *code = malloc(MAX_SIZE * sizeof(char));
    size_t len = fread(code, sizeof(char), MAX_SIZE, stdin);

    fwrite(code, sizeof(char), len, source);
    fclose(source);

    char *args[] = {CC, SRC, "-lm", "-o", OUT, NULL};
    int pid, status, spawn_ret;
    if (execvp(CC, args) != -1)
    {
        printf("1: success execvp!\n");
        if (wait(NULL) != -1)
        {
            printf("0: success!\n");
            return 0;
        }
    }
    printf("4: %m\n", errno);
    return 1;
}