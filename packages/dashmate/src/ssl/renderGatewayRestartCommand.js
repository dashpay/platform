import renderConfigFlag from '../util/renderConfigFlag.js';

/**
 * The command that restarts the gateway, so that it mounts its certificate
 * files again
 *
 * `restart --platform` stops Platform by its Compose profiles. With the
 * quorum list enabled that stops Core too, without the DKG check a masternode
 * gets before Core stops, and starting Platform alone then fails, so the
 * whole node is restarted instead.
 *
 * @param {Config} config
 * @return {string}
 */
export default function renderGatewayRestartCommand(config) {
  const flag = renderConfigFlag(config.getName());

  return config.get('platform.quorumList.enabled')
    ? `dashmate restart ${flag}`
    : `dashmate restart --platform ${flag}`;
}
