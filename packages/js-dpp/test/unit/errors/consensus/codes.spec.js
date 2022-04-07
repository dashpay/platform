const path = require('path');
const fs = require('fs');
const AbstractStateError = require('../../../../lib/errors/consensus/state/AbstractStateError');
const AbstractBasicError = require('../../../../lib/errors/consensus/basic/AbstractBasicError');
const codes = require('../../../../lib/errors/consensus/codes');

const getAllFiles = (dirPath, arrayOfFiles) => {
  const files = fs.readdirSync(dirPath);

  // eslint-disable-next-line no-param-reassign
  arrayOfFiles = arrayOfFiles || [];

  files.forEach((file) => {
    if (fs.statSync(`${dirPath}/${file}`).isDirectory()) {
      // eslint-disable-next-line no-param-reassign
      arrayOfFiles = getAllFiles(`${dirPath}/${file}`, arrayOfFiles);
    } else if (file.slice(-3) === '.js') {
      arrayOfFiles.push(path.join(dirPath, '/', file));
    }
  });

  return arrayOfFiles;
};

function isChildOf(classToCheck, parentClass) {
  if (!classToCheck || !classToCheck.prototype) {
    return false;
  }

  if (classToCheck.prototype instanceof parentClass) {
    return true;
  }

  return isChildOf(classToCheck.prototype, parentClass);
}

describe('Consensus error codes', () => {
  // Skip the tests for browsers
  if (global.window !== undefined) {
    return;
  }

  const normalizedPath = path.join(__dirname, '../../../../lib/errors/');
  const allFiles = getAllFiles(normalizedPath);

  allFiles.forEach((fileName) => {
    // eslint-disable-next-line global-require,import/no-dynamic-require
    const ErrorClass = require(fileName);

    if (
      (isChildOf(ErrorClass, AbstractStateError) || (isChildOf(ErrorClass, AbstractBasicError)))
      && !ErrorClass.name.startsWith('Abstract')
    ) {
      it(`should have error code defined for ${ErrorClass.name}`, () => {
        const hasErrorCode = !!Object.values(codes)
          .find((ErrorClassWithCode) => ErrorClassWithCode === ErrorClass);

        expect(hasErrorCode).to.be.true();
      });
    }
  });
});
