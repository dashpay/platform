const Node = require('./lib/node/node');
const config = require('./lib/config');
const { isPortTaken } = require('./lib/utils');

const { rep, pub, pubKey } = config.node.rep;

async function prepareReplier() {
  const taken = await isPortTaken(rep.port);
  if (taken) {
    rep.port++;
    await prepareReplier();
  }
  return true;
}
async function preparePublisher() {
  const taken = await isPortTaken(pub.port);
  if (taken) {
    pub.port++;
    await preparePublisher();
  }
  return true;
}

async function starter() {
  await preparePublisher();
  await prepareReplier();
  try {
    const node = new Node({
      debug: true,
      rep,
      pub,
      pubKey: pubKey + rep.port, // Just in order to make it unique. TO BE REMOVED TODO
    });
  } catch (e) {
    console.log('Cannot start node...');
    console.error(e);
  }
}

starter();

process.on('uncaughtException', (err) => {
  console.log(err);
});
