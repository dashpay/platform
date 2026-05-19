import { inspect } from 'util';

/**
 * @param {Error} e
 */
function throwGrpcErrorWithMetadata(e) {
  if (e.metadata) {
    e.message = `${e.message}\n\nMetadata:\n${inspect(e.metadata.getMap())}`;
  }

  throw e;
}

export default throwGrpcErrorWithMetadata;
