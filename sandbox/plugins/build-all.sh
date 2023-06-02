for entry in ./*
do
  if [ -f "$entry"/spec.toml ]; then
    echo "building plugin $entry"
    sh -c "cd \"$entry\" && sh build.sh"
  fi
done