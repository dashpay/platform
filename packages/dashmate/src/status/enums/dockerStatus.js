const ServiceStatusEnum = {
  // all possible Docker statuses
  created: 'created',
  restarting: 'restarting',
  running: 'running',
  removing: 'removing',
  exited: 'exited',
  dead: 'dead',
};

module.exports = ServiceStatusEnum;
