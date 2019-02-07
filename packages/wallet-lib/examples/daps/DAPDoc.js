const crypto = require('crypto'),
  fs = require('fs');
const { DAP } = require('../../src/plugins');

const registerDAP = () => {

};
class DAPDoc extends DAP {
  async notarizeDocument(path) {
    const buffer = await this.getBufferFromPath(path);
    const notarizedHash = this.getHashFromBuffer(buffer);
    try {
      const notarize = await this.notarize(notarizedHash);
      return {
        hash: notarizedHash,
        result: notarize,
      };
    } catch (e) {
      return {
        hash: notarizedHash,
        error: e,
      };
    }
  }

  async getBufferFromPath(path) {
    const fileStream = fs.createReadStream(path);
    return new Promise((res, rej) => {
      fileStream.on('readable', async () => {
        const buffer = await fileStream.read();
        if (buffer) {
          return res(buffer);
        }

        rej(`Failed to get buffer of ${path}`);
      });
    });
  }

  getHashFromBuffer(buff) {
    const cryptoHash = crypto.createHash('sha256');
    cryptoHash.update(buff);
    return cryptoHash.digest('hex');
  }

  notarize(hash) {
    // TODO : Here the logic would be, to store the hash into a state transition.
    throw new Error('Not implemented');
  }
}

module.exports = DAPDoc;
