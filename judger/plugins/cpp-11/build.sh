mkdir -p rootfs
sudo docker build -t cpp-11-mdoj-plugin .
sudo docker export $(sudo docker create cpp-11-mdoj-plugin) | tar -C rootfs -xvf - > /dev/null