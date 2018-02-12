# MN Bootstrap

### Pre-requisites to be Installed

* [Python](https://www.python.org/downloads/)
* [docker](https://docs.docker.com/engine/installation/) (version 17.04.0+)
* docker-compose (`pip install -U docker-compose`)
* awscli - for private Docker registry aka "ECR" (`pip install -U awscli`)

### Setup

0. Clone this repo & cd to the directory:

```
git clone git@github.com:dashevo/mn-bootstrap.git ./mn-bootstrap
cd mn-bootstrap
```

1. set AWS key vars & login to AWS ECR

```
. ./.env
$(aws ecr get-login --no-include-email)
```

You can ignore the warning that states: `WARNING! Using --password via the CLI is insecure. Use --password-stdin.`

### Using mn-bootstrap.sh for regtest

mn-bootstrap provides a wrapper around docker-compose to make using different networks
easier. It is called this way:

```bash
$ ./mn-bootstrap.sh <network> <compose_args...>
```

To bootstrap with regtest as network use:

```bash
$ ./mn-bootstrap.sh regtest up -d
```

The argument `-d` is is used to start everything in the background. User `logs`
to view the logs in the foreground:

```bash
$ ./mn-bootstrap.sh regtest logs
```

If you want to call dash-cli inside of the dashd container, you can use the `dash-cli.sh` wrapper

```bash
$ ./dash-cli.sh regtest getinfo
```

To shut down everything, use:

```bash
$ ./mn-bootstrap regtest down
```

To delete all containers and node data, use:

```bash
$ ./mn-bootstrap.sh regtest rm -fv
# sudo is needed because docker will create volumes with different owner then your user
$ sudo rm -rf ./data/core-regtest
```

### Connecting mn-bootstrap to devnet

To connect mn-bootstrap to an existing devnet, you'll have to do some preparations first. You'll have to open the devnet
dashd port on your router, prepare a MN privkey and edit `devnet-dashevo1.env`. It is also recommended to have a dash-qt
node connected to the same devnet to make working with it easier.

1. Connect a normal dash-qt wallet to the devnet

Use the example configuration provided in `examples/dash-qt-devnet`.

It is important that you use a version of dash-qt that is compatible to the used devnet.
If you connect to a public devnet, use the latest released version (>=0.12.3, which is not released at time of writing)
or compile it by yourself from [dashpay/dash](https://github.com/dashpay/dash) (branch develop).

For the private dashevo devnet, use a self compiled binary from [dashevo/dash](https://github.com/dashevo/dash)

2. Fund the MN collateral

Use your Qt-Wallet to generate an address and send 1000 Dash to it using the Devnet Faucet
(please ask in Slack for the URL of the faucet as we don't have a final URL atm)

After confirmation of the TX, use `masternode outputs` (from debug console or dash-cli) to find the collateral TX and index

3. Generate a MN privkey in the Qt-Wallet

Use the debug console or dash-cli to generate a MN privkey by calling `masternode genprivkey`

4. Update `networks/devnet-dashevo1.env` and `masternodes.conf`

Put the generated MN privkey and collateral TX+index into `masternodes.conf` of your Qt-Wallet. Restart your Qt-Wallet afterwards.

Now put the MN privkey into `networks/devnet-dashevo1.env` and update your public/external IP as well.

5. Start up mn-bootstrap

Use the instructions from `Using mn-bootstrap.sh for regtest` but with `devnet-dashevo1` as network parameter

6. Start the MN from the Qt-Wallet

Use the debug console or dash-cli to call `masternode start-missing`

7. Done

Now watch the logs of mn-bootstrap to see if the MN is working properly.
