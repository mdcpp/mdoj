mkdir -p rootfs
docker build -t c-11-mdoj-plugin .
docker export $(docker create c-11-mdoj-plugin) > c-11.lang
mv c-11.lang ..