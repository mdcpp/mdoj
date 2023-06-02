mkdir -p rootfs
docker build -t lua-5.2-mdoj-plugin .
docker export $(docker create lua-5.2-mdoj-plugin) | tar -C rootfs -xvf -
