const ServiceStatusEnum = {
  // all possible Docker statuses
  created: 'created',
  restarting: 'restarting',
  running: 'running',
  removing: 'removing',
  exited: 'exited',
  dead: 'dead',
  // extension (used in status command)
  syncing: 'syncing',
  not_started: 'not_started',
  wait_for_core: 'wait_for_core',
};

module.exports = ServiceStatusEnum;
