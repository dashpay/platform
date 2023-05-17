#!/bin/sh
USER_ID=${LOCAL_UID:?err}
GROUP_ID=${LOCAL_GID:?err}
DOCKER_GROUP_ID=$(stat -c %g /var/run/docker.sock)
USERNAME=dashmate
GROUP=docker

# Only if we are root

if [ "$(id -u)" -eq "0" ]; then
  # check if user with our uid exists in the system
  if [ ! $(getent passwd $USER_ID | grep $USER_ID -q) ]; then
    echo "Creating user $USERNAME ($USER_ID)"
    adduser -u $USER_ID -D -H $USERNAME
  else
    USERNAME=$(getent passwd $USER_ID | cut -d: -f1)
    echo "User exist: $USERNAME $USER_ID"
  fi

  # check if docker group exists in the container
  if [ -z $(getent group $DOCKER_GROUP_ID) ]; then
    echo "Creating group $DOCKER_GROUP_ID $GROUP"
    addgroup -g $DOCKER_GROUP_ID $GROUP
  else
    GROUP=$(getent group $DOCKER_GROUP_ID | cut -d: -f1)
    echo "Group exist: $GROUP $DOCKER_GROUP_ID"
  fi

  # check if our user belongs to docker group
  if [ ! $(id -nG $USERNAME | grep -q $GROUP) ]; then
    echo "Adding $USERNAME to group $GROUP"
    adduser $USERNAME $GROUP
  fi

  echo "Starting with: USERNAME: $USERNAME, UID: $USER_ID, GID: $GROUP_ID, USER: $USERNAME, GROUP: $GROUP"

  exec su - $USERNAME -c "cd /platform;DASHMATE_HELPER=1 $*"
else
  cd /platform
  DASHMATE_HELPER=1 $*
fi
