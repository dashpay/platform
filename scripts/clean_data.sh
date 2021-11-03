echo "Removing all docker containers and volumes..."
docker rm -f -v $(docker ps -a -q) || true

docker system prune -f --volumes

echo "Remove dashmate configuration..."
rm -rf ~/.dashmate/