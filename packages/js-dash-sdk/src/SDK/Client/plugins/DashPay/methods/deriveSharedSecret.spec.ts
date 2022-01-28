import "mocha";
import { expect } from "chai";
import { deriveSharedSecret } from "./deriveSharedSecret";

describe("DashPayPlugin - encryptPublicKey", () => {
  it("should encrypt a publicKey", function () {
    const expectedSharedSecret =
      "0ec54a54b97988862cadf92b0f09337f9aabee0ecfbedaac23a635264a3a39e5";
    const senderPrivateKeyBuffer = Buffer.from(
      "2fc4145c8b7a871c42e32733a83c36f9b0d0eb646f40e53cb9ae0f48669ab0d7",
      "hex"
    );
    const receiverPublicKeyBuffer = Buffer.from(
      "03a9f4f3c1409fa84da275efff6ff2203203db5d5c784d543a86e1b2f0bf4c3e8f",
      "hex"
    );
    const sharedSecret = deriveSharedSecret(
      senderPrivateKeyBuffer,
      receiverPublicKeyBuffer
    );
    expect(sharedSecret).to.equal(expectedSharedSecret);
  });
});
