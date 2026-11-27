/**
 * Select every Platform service of the config, whether the config enables it
 * or not
 *
 * Commands that act on Platform only name these services rather than select
 * their profiles. A profile selection leaves Core out of the Compose model, and
 * a Platform service that depends on Core (the quorum list) then makes the
 * project invalid: Compose fails, or for rm and stop acts on the whole project,
 * Core included. Every profile is still enabled next to the names, because some
 * Compose versions (2.10 to 2.12) skip a named service whose profile is not
 * enabled. With every service in the model, Compose acts on the named ones
 * only.
 *
 * @typedef {function} selectPlatformServices
 * @param {getServiceList} getServiceList
 * @param {Config} config
 * @return {{ serviceNames: string[], profiles: string[] }}
 */
export default function selectPlatformServices(getServiceList, config) {
  const services = getServiceList(config, { allProfiles: true });

  const serviceNames = services
    .filter(({ profiles }) => profiles.some((profile) => profile.startsWith('platform')))
    .map(({ name }) => name);

  // Compose acts on every service of the project when given no names.
  if (serviceNames.length === 0) {
    throw new Error('No Platform services found in the Compose files');
  }

  const profiles = Array.from(new Set(services.flatMap((service) => service.profiles)));

  return { serviceNames, profiles };
}
