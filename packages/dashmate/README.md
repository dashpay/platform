# MN Bootstrap

### Pre-requisites to be Installed

* [Python](https://www.python.org/downloads/)
* [docker](https://docs.docker.com/engine/installation/) (version 17.04.0+)
* docker-compose (`pip install -U docker-compose`)
* awscli - for private Docker registry aka "ECR" (`pip install -U awscli`)

### Setup

1. set AWS key vars & login to AWS ECR

```
. ./.env
$(aws ecr get-login --no-include-email)
```

You can ignore the warning that states: `WARNING! Using --password via the CLI is insecure. Use --password-stdin.`

2. Bootstrap / start up services

To view output of services in the foreground:

```
docker-compose up
```

... you can also choose to throw the services into the background (e.g. daemonize them):

```
docker-compose up -d
```

If you do this, you will eventually want to shut them down:

```
docker-compose down
```
