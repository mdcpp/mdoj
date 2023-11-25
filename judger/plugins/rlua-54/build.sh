mkdir -p rootfs
docker build --build-arg ARCH=$(uname -m) -t rlua-54-mdoj-plugin .
docker export $(docker create rlua-54-mdoj-plugin) | tar -C rootfs -xvf -
