mkdir -p rootfs
docker build -t cpp-11-mdoj-plugin .
docker export $(docker create cpp-11-mdoj-plugin) | tar -C rootfs -xvf - > /dev/null