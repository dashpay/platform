const IPFSApi = require('ipfs-api');

module.exports = new IPFSApi(process.env.STORAGE_IPFS_MULTIADDR);
