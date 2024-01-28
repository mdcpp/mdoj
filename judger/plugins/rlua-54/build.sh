mkdir -p rootfs
sudo docker build --build-arg ARCH=$(uname -m) -t rlua-54-mdoj-plugin .
sudo docker export $(sudo docker create rlua-54-mdoj-plugin) | tar -C rootfs -xvf - > /dev/null
