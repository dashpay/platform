const fetch = require('../src/fetch');

const contractId = process.argv[2];

fetch(contractId)
  // eslint-disable-next-line no-console
  .then(contract => console.dir(contract));
