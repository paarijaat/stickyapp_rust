#!/bin/bash

REGISTRY=${REGISTRY:-paarijaat:paari@paarijaat-debian-vm:5000}
for i in stickyapp_rust
do
  if [ ! -z $1 ]
  then
    t=$1
    digest=$(curl -s -H 'Accept: application/vnd.docker.distribution.manifest.v2+json' https://$REGISTRY/v2/paarijaat/$i/manifests/$t|jq -r '.config.digest')
    if [ "$digest" != "" ]
    then
      echo -n "paarijaat/$i:$t	"
      echo $digest
    else
      echo -n "Not found: $i:$t" >2
    fi
  else
    for t in $(curl -s https://$REGISTRY/v2/paarijaat/$i/tags/list|jq -r '.tags[]')
    do
      echo -n "paarijaat/$i:$t	"
      curl -s -H 'Accept: application/vnd.docker.distribution.manifest.v2+json' https://$REGISTRY/v2/paarijaat/$i/manifests/$t|jq -r '.config.digest'
    done
  fi
done

