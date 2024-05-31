mkdir -p rootfs
docker build -t cpp-11-mdoj-plugin .
docker export $(docker create cpp-11-mdoj-plugin) > cpp-11.lang
mv cpp-11.lang ..