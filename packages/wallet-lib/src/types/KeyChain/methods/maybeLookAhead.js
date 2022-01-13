function maybeLookAhead() {
  const { lookAheadOpts } = this;
  const generatedPaths = [];

  if (Object.keys(lookAheadOpts.paths).length === 0) {
    return generatedPaths;
  }

  const usedPaths = [...this.issuedPaths.entries()]
    .filter(([, el]) => el.isUsed === true)
    .map(([path]) => path);

  const sortedUsedPathByBase = {};

  usedPaths
    .forEach((usedPath) => {
      const splitted = usedPath.split('/');
      // Removes the index to sort which and how many base path has been generated
      const basePath = splitted.splice(0, splitted.length - 1).join('/');
      if (!sortedUsedPathByBase[basePath]) sortedUsedPathByBase[basePath] = [];
      sortedUsedPathByBase[basePath].push(usedPath);
    });

  const lastUsedIndexes = {};
  const lastGeneratedIndexes = {};

  Object
    .entries(lookAheadOpts.paths)
    .forEach(([basePath]) => {
      lastUsedIndexes[basePath] = -1;
      lastGeneratedIndexes[basePath] = -1;
    });

  Object
    .entries(sortedUsedPathByBase)
    .forEach(([basePath, basePaths]) => {
      // Sorting by index is also needed as the user might have manually issue a key
      // and set it up to watched or used outside of lookAhead bounds
      const sortedBasePaths = basePaths.sort((a, b) => a.split('/').splice(-1) - b.split('/').splice(-1));

      let prevIndex;
      sortedBasePaths.forEach((path) => {
        const addressData = this.issuedPaths.get(path);

        const currentIndex = parseInt(path.split('/').splice(-1), 10);

        if (addressData.isUsed) {
          lastUsedIndexes[basePath] = currentIndex;
        }

        lastGeneratedIndexes[basePath] = currentIndex;
        prevIndex = currentIndex;
      });
    });

  const isWatched = lookAheadOpts.isWatched || false;

  Object
    .entries(lastGeneratedIndexes)
    .forEach(([basePath]) => {
      const lastUsedAndLastGenGap = lastGeneratedIndexes[basePath] - lastUsedIndexes[basePath];
      const pathAmountToGenerate = lookAheadOpts.paths[basePath] - lastUsedAndLastGenGap;

      if (pathAmountToGenerate > 0) {
        const lastIndex = lastGeneratedIndexes[basePath];
        const lastIndexToGenerate = lastIndex + pathAmountToGenerate;

        if (lastIndexToGenerate > lastIndex) {
          for (
            let index = lastIndex + 1;
            index <= lastIndexToGenerate;
            index += 1) {
            const timeNow = +new Date();
            const pathData = this.getForPath(`${basePath}/${index}`, { isWatched });
            if (pathData.issuedTime >= timeNow) {
              generatedPaths.push(pathData);
            }
          }
        }
      }
    });
  return generatedPaths;
}
module.exports = maybeLookAhead;
