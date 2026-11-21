import basicSsl from '@vitejs/plugin-basic-ssl';

// Phones only allow camera access over HTTPS, so the dev server uses a self-signed cert on the LAN.
export default { plugins: [basicSsl()], server: { host: true } };
