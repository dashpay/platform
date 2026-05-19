import DAPIClient from '@dashevo/dapi-client';

import _ from 'lodash';

import DAPIClientTransport from './DAPIClientTransport/DAPIClientTransport.js';

/**
 *
 * @param {DAPIClientOptions|Transport|DAPIClientTransport} options
 * @returns {Transport|DAPIClientTransport}
 */
function createTransportFromOptions(options) {
  if (!_.isPlainObject(options)) {
    // Return transport instance
    return options;
  }

  const client = new DAPIClient(options);

  return new DAPIClientTransport(client);
}

export default createTransportFromOptions;