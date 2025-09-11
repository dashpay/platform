import init, * as sdk from '../../dist/sdk.js';

describe('WasmSdkBuilder', () => {
  before(async () => {
    await init();
  });

  it('WasmSdkBuilder static methods exist', () => {
    expect(sdk.WasmSdkBuilder).to.be.a('function');
    expect(sdk.WasmSdkBuilder.getLatestVersionNumber).to.be.a('function');
    expect(sdk.WasmSdkBuilder.new_mainnet).to.be.a('function');
    expect(sdk.WasmSdkBuilder.new_testnet).to.be.a('function');
    expect(sdk.WasmSdkBuilder.new_mainnet_trusted).to.be.a('function');
    expect(sdk.WasmSdkBuilder.new_testnet_trusted).to.be.a('function');
  });

  it('builds testnet builder and sets version', async () => {
    let builder = sdk.WasmSdkBuilder.new_testnet();
    expect(builder).to.be.ok;
    // note: builder methods consume and return a new builder
    builder = builder.with_version(1);
    const built = await builder.build();
    expect(built).to.be.ok;
    built.free();
  });

  it('applies custom settings (timeouts, retries, ban flag)', async () => {
    // with_settings(connect_timeout_ms, timeout_ms, retries, ban_failed_address)
    let builder = sdk.WasmSdkBuilder.new_testnet();
    builder = builder.with_settings(5000, 10000, 3, true);
    const built = await builder.build();
    expect(built).to.be.ok;
    built.free();
  });
});
