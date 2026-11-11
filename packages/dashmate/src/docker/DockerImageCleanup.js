const MAX_IMAGE_AGE_MS = 14 * 24 * 60 * 60 * 1000;

// Current and historical first-party repositories shipped by dashmate. In
// particular, dapi must remain here after its replacement by rs-dapi: the first
// cleanup needs to reclaim years of old images, without a prior inventory.
const IMAGE_REPOSITORIES = new Set([
  'dashpay/dashd',
  'dashpay/drive',
  'dashpay/dapi',
  'dashpay/rs-dapi',
  'dashpay/tenderdash',
  'dashpay/dashmate-helper',
  'dashpay/envoy',
  'dashpay/quorum-list-server',
  'dashpay/insight',
  'dashpay/insight-api',
]);

// Labels used by the Platform Dockerfile since before mainnet launch. These
// identify old dangling images even when both tags and repository digests have
// disappeared. Do not use the generic maintainer label alone (e.g. test-suite).
const IMAGE_DESCRIPTIONS = new Set([
  'Drive ABCI Rust',
  'DAPI Node.JS',
  'Dashmate Helper Node.JS',
  'Dash Platform API (DAPI) - Rust Implementation',
]);

/**
 * @param {Object} image
 * @returns {boolean}
 */
function isDashmateImage(image) {
  const references = [...(image.RepoTags ?? []), ...(image.RepoDigests ?? [])]
    .filter((reference) => !reference.startsWith('<none>'));
  if (references.length > 0) {
    // An unrelated alias on the same ID protects the entire image. Never
    // interpret an arbitrary registry's dashpay namespace as Docker Hub.
    return references.every((reference) => {
      const repository = reference.replace(/^(?:docker\.io|index\.docker\.io)\//, '')
        .split('@')[0].replace(/:[^/]+$/, '');
      return IMAGE_REPOSITORIES.has(repository);
    });
  }
  const labels = image.Config?.Labels ?? {};
  return labels.maintainer === 'Dash Developers <dev@dash.org>'
    && IMAGE_DESCRIPTIONS.has(labels.description);
}

/**
 * Used under UpdateCommand's configuration lock. Discover historical images
 * directly from Docker; no inventory or previous dashmate update is required.
 */
export default class DockerImageCleanup {
  /**
   * @param {Docker} docker
   * @param {getServiceList} getServiceList
   * @param {generateEnvs} generateEnvs
   * @param {getConfiguredDockerImages} getConfiguredDockerImages
   */
  constructor(docker, getServiceList, generateEnvs, getConfiguredDockerImages) {
    this.docker = docker;
    this.getServiceList = getServiceList;
    this.generateEnvs = generateEnvs;
    this.getConfiguredDockerImages = getConfiguredDockerImages;
  }

  /**
   * @param {string} reference
   * @returns {Promise<Object|null>}
   */
  async inspect(reference) {
    try {
      return await this.docker.getImage(reference).inspect();
    } catch (error) {
      if (error.statusCode === 404) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Retain the immediate rollback IDs before mutable tags move.
   *
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async prepare(config) {
    const { ID: daemonId } = await this.docker.info();
    if (!daemonId) {
      throw new Error('Cannot identify the Docker daemon');
    }
    const rollbackIds = new Set();
    const references = new Set(this.getServiceList(config).map(({ image }) => image));
    for (const reference of references) {
      const image = await this.inspect(reference);
      if (image) {
        rollbackIds.add(image.Id);
      }
    }

    const project = this.generateEnvs(config).COMPOSE_PROJECT_NAME;
    const containers = await this.docker.listContainers({ all: true });
    for (const container of containers) {
      if (container.Labels?.['com.docker.compose.project'] === project
        && container.Labels?.['org.dashmate.service.title']) {
        rollbackIds.add(container.ImageID);
      }
    }
    return { daemonId, rollbackIds, references };
  }

  /**
   * Cleanup only after every pull and the command's certificate gate succeeded.
   * Docker creation time is build age, NOT time since last pull/use.
   *
   * @param {Object} snapshot
   * @returns {Promise<{deleted: string[], skipped: string[]}>}
   */
  async cleanup({ daemonId, rollbackIds, references }) {
    const { ID: currentDaemonId } = await this.docker.info();
    if (currentDaemonId !== daemonId) {
      throw new Error('Docker daemon changed during the update');
    }
    const protectedIds = new Set(rollbackIds);
    for (const reference of references) {
      const image = await this.inspect(reference);
      if (!image) {
        throw new Error('An updated Docker image is no longer available');
      }
      protectedIds.add(image.Id);
    }

    // Resolve every config, including disabled profiles and overrides, before
    // deleting anything. A missing local image has no ID to protect; other
    // Docker/parse errors abort cleanup instead of weakening protection.
    for (const reference of this.getConfiguredDockerImages()) {
      const image = await this.inspect(reference);
      if (image) {
        protectedIds.add(image.Id);
      }
    }

    const cutoff = Date.now() - MAX_IMAGE_AGE_MS;
    const result = { deleted: [], skipped: [] };
    const images = await this.docker.listImages({ all: true });
    for (const { Id: id } of images) {
      if (protectedIds.has(id)) {
        continue;
      }
      // Re-inspect: tags/labels may have changed since the image listing.
      const image = await this.inspect(id);
      if (!image || !isDashmateImage(image)) {
        continue;
      }
      const created = Date.parse(image.Created);
      if (!Number.isFinite(created) || created >= cutoff) {
        continue;
      }

      // Include stopped and non-dashmate containers. Non-force removal also
      // closes the race with a container created after this check.
      const containers = await this.docker.listContainers({ all: true });
      if (containers.some((container) => container.ImageID === id)) {
        result.skipped.push(id);
        continue;
      }
      try {
        await this.docker.getImage(id).remove({ force: false, noprune: true });
        result.deleted.push(id);
      } catch (error) {
        if (error.statusCode === 409) {
          result.skipped.push(id);
        } else if (error.statusCode !== 404) {
          throw error;
        }
      }
    }
    return result;
  }
}
