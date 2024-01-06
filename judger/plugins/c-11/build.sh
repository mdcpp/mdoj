mkdir -p rootfs
sudo docker build -t c-11-mdoj-plugin .
sudo docker export $(sudo docker create c-11-mdoj-plugin) | tar -C rootfs -xvf -