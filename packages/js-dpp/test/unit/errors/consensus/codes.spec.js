const path = require('path');
const fs = require('fs');
const codes = require('../../../../lib/errors/consensus/codes');
const AbstractConsensusError = require('../../../../lib/errors/consensus/AbstractConsensusError');
const AbstractBasicError = require('../../../../lib/errors/consensus/basic/AbstractBasicError');
const AbstractSignatureError = require('../../../../lib/errors/consensus/signature/AbstractSignatureError');
const AbstractFeeError = require('../../../../lib/errors/consensus/fee/AbstractFeeError');
const AbstractStateError = require('../../../../lib/errors/consensus/state/AbstractStateError');

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

const errorClasses = Object.values(codes).map((ErrorClass) => ErrorClass);
const errorClassDuplicates = errorClasses.filter((item, index) => (
  errorClasses.indexOf(item) !== index
));

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
      (isChildOf(ErrorClass, AbstractConsensusError)) && !ErrorClass.name.startsWith('Abstract')
    ) {
      context(ErrorClass.name, () => {
        let code;
        let AssignedErrorClass;

        beforeEach(() => {
          const result = Object.entries(codes)
            .find(([, ErrorClassWithCode]) => ErrorClassWithCode === ErrorClass);

          if (result) {
            code = Number(result[0]);
            // eslint-disable-next-line prefer-destructuring
            AssignedErrorClass = result[1];
          }
        });

        it('should have error code defined', () => {
          expect(AssignedErrorClass).to.exist();
        });

        it('should have been define in the correct code range', () => {
          if (isChildOf(ErrorClass, AbstractBasicError)) {
            expect(code).to.be.above(999);
            expect(code).to.be.below(2000);
          } else if (isChildOf(ErrorClass, AbstractSignatureError)) {
            expect(code).to.be.above(1999);
            expect(code).to.be.below(3000);
          } else if (isChildOf(ErrorClass, AbstractFeeError)) {
            expect(code).to.be.above(2999);
            expect(code).to.be.below(4000);
          } else if (isChildOf(ErrorClass, AbstractStateError)) {
            expect(code).to.be.above(3999);
            expect(code).to.be.below(5000);
          }
        });

        it('should not have duplicates', () => {
          expect(errorClassDuplicates).to.not.include(ErrorClass);
        });
      });
    }
  });
});
