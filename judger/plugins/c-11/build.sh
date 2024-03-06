mkdir -p rootfs
docker build -t c-11-mdoj-plugin .
docker export $(docker create c-11-mdoj-plugin) | tar -C rootfs -xvf - > /dev/null