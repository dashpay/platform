# MN Bootstrap

### Pre-requisites to be Installed

* [Python](https://www.python.org/downloads/)
* [docker](https://docs.docker.com/engine/installation/)
* docker-compose (`pip install -U docker-compose`)
* awscli - for private Docker registry aka "ECR" (`pip install -U awscli`)

### Setup

1. set AWS key vars & login to AWS ECR

```
. ./.env
$(aws ecr get-login --no-include-email)
```

You can ignore the warning about `--stdin`, blah.

2. Bootstrap

```
docker-compose up
```
