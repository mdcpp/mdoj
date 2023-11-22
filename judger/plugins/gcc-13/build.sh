mkdir -p rootfs
docker build -t gcc-13-mdoj-plugin .
docker export $(docker create gcc-13-mdoj-plugin) | tar -C rootfs -xvf -
