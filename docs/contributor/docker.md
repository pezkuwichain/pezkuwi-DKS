# Using Containers

The following commands should work no matter if you use Docker or Podman. In general, Podman is recommended. All
commands are "engine neutral" so you can use the container engine of your choice while still being able to copy/paste
the commands below.

Let's start defining Podman as our engine:
```
ENGINE=podman
```

If you prefer to stick with Docker, use:
```
ENGINE=docker
```

## The easiest way

The easiest/faster option to run Pezkuwi in Docker is to use the latest release images. These are small images that use
the latest official release of the Pezkuwi binary, pulled from our Debian package.

**_The following examples are running on zagros chain and without SSL. They can be used to quick start and learn how
Pezkuwi needs to be configured. Please find out how to secure your node, if you want to operate it on the internet. Do
not expose RPC and WS ports, if they are not correctly configured._**

Let's first check the version we have. The first time you run this command, the Pezkuwi docker image will be
downloaded. This takes a bit of time and bandwidth, be patient:

```bash
$ENGINE run --rm -it parity/pezkuwi:latest --version
```

You can also pass any argument/flag that Pezkuwi supports:

```bash
$ENGINE run --rm -it parity/pezkuwi:latest --chain zagros --name "PolkaDocker"
```

## Examples

Once you are done experimenting and picking the best node name :) you can start Pezkuwi as daemon, exposes the Pezkuwi
ports and mount a volume that will keep your blockchain data locally. Make sure that you set the ownership of your local
directory to the Pezkuwi user that is used by the container.

Set user id 1000 and group id 1000, by running `chown 1000.1000 /my/local/folder -R` if you use a bind mount.

To start a Pezkuwi node on default rpc port 9933 and default p2p port 30333 use the following command. If you want to
connect to rpc port 9933, then must add Pezkuwi startup parameter: `--rpc-external`.

```bash
$ENGINE run -d -p 30333:30333 -p 9933:9933 \
    -v /my/local/folder:/pezkuwi \
    parity/pezkuwi:latest \
    --chain zagros --rpc-external --rpc-cors all \
    --name "PolkaDocker
```

If you also want to expose the webservice port 9944 use the following command:

```bash
$ENGINE run -d -p 30333:30333 -p 9933:9933 -p 9944:9944 \
    -v /my/local/folder:/pezkuwi \
    parity/pezkuwi:latest \
    --chain zagros --ws-external --rpc-external --rpc-cors all --name "PolkaDocker"
```

## Using Docker compose

You can use the following docker-compose.yml file:

```bash
version: '2'

services:
  pezkuwi:
    container_name: pezkuwi
    image: parity/pezkuwi
    ports:
      - 30333:30333 # p2p port
      - 9933:9933 # rpc port
      - 9944:9944 # ws port
      - 9615:9615 # Prometheus port
    volumes:
      - /my/local/folder:/pezkuwi
    command: [
      "--name", "PolkaDocker",
      "--ws-external",
      "--rpc-external",
      "--prometheus-external",
      "--rpc-cors", "all"
    ]
```

With following `docker-compose.yml` you can set up a node and use `pezkuwi-js-apps` as the front end on port 80. After
starting the node use a browser and enter your Docker host IP in the URL field: _<http://[YOUR_DOCKER_HOST_IP]>_

```bash
version: '2'

services:
  pezkuwi:
    container_name: pezkuwi
    image: parity/pezkuwi
    ports:
      - 30333:30333 # p2p port
      - 9933:9933 # rpc port
      - 9944:9944 # ws port
      - 9615:9615 # Prometheus port
    command: [
      "--name", "PolkaDocker",
      "--ws-external",
      "--rpc-external",
      "--prometheus-external",
      "--rpc-cors", "all"
    ]

  pezkuwiui:
    container_name: pezkuwiui
    image: jacogr/pezkuwi-js-apps
    environment:
      - WS_URL=ws://[YOUR_DOCKER_HOST_IP]:9944
    ports:
      - 80:80
```

## Limiting Resources

Chain syncing will utilize all available memory and CPU power your server has to offer, which can lead to crashing.

If running on a low resource VPS, use `--memory` and `--cpus` to limit the resources used. E.g. To allow a maximum of
512MB memory and 50% of 1 CPU, use `--cpus=".5" --memory="512m"`. Read more about limiting a container's resources
[Docker resource constraints documentation](https://docs.docker.com/config/containers/resource_constraints).


## Build your own image

There are 3 options to build a Pezkuwi container image:
- using the builder image
- using the injected "Debian" image
- using the generic injected image

### Builder image

To get up and running with the smallest footprint on your system, you may use an existing Pezkuwi Container image.

You may also build a Pezkuwi container image yourself (it takes a while...) using the container specs
`docker/dockerfiles/pezkuwi/pezkuwi_builder.Dockerfile`.

### Debian injected

The Debian injected image is how the official Pezkuwi container image is produced. It relies on the Debian package that
is published upon each release. The Debian injected image is usually available a few minutes after a new release is
published. It has the benefit of relying on the GPG signatures embedded in the Debian package.

### Generic injected

For simple testing purposes, the easiest option for Pezkuwi and also random binaries, is to use the
`binary_injected.Dockerfile` container spec. This option is less secure since the injected binary is not checked at all
but it has the benefit to be simple. This option requires to already have a valid `pezkuwi` binary, compiled for Linux.

This binary is then simply copied inside the `parity/base-bin` image.

## Reporting issues

If you run into issues with Pezkuwi when using docker, please run the following command (replace the tag with the
appropriate one if you do not use latest):

```bash
$ENGINE run --rm -it parity/pezkuwi:latest --version
```

This will show you the Pezkuwi version as well as the git commit ref that was used to build your container. You can now
paste the version information in a [new issue](https://github.com/pezkuwichain/pezkuwi-sdk/issues/new/choose).
