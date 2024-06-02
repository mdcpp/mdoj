mkdir -p rootfs
docker build --build-arg ARCH=$(uname -m) -t rlua-54-mdoj-plugin .
docker export $(docker create rlua-54-mdoj-plugin) > rlua-54.lang
mv rlua-54.lang ..