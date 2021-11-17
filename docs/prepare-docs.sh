# Consolidate package docs into one folder for building documentation
cp -r ../packages/js-dapi-client/docs/ ./docs/DAPI-Client
mv ./docs/DAPI-Client/_sidebar.md ./docs/DAPI-Client/Overview.md
cp -r ../packages/js-dpp/docs/ ./docs/Dash-Platform-Protocol
mv ./docs/Dash-Platform-Protocol/_sidebar.md ./docs/Dash-Platform-Protocol/Overview.md
cp -r ../packages/js-dash-sdk/docs/ ./docs/SDK
mv ./docs/SDK/_sidebar.md ./docs/SDK/Overview.md
cp -r ../packages/wallet-lib/docs/ ./docs/Wallet-library
mv ./docs/Wallet-library/_sidebar.md ./docs/Wallet-library/Overview.md
