for entry in ./*
do
  if [ -f "$entry"/spec.toml ]; then
    echo "exporting plugin $entry"
    sh -c "tar -czvf \"../plugins-out/$entry.tar.gz\" \"$entry/spec.toml\" \"$entry/rootfs\""
  fi
done