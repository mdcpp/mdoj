import {
  generateCredential,
  serverAddress,
  TokenSetClient,
  UserSetClient,
} from "../load.ts";

import { TokenInfo } from "../codegen/proto/backend.ts";

export async function setup(t: Deno.TestContext) {
  await t.step("setup", async () => {
    let client = new TokenSetClient(serverAddress, await generateCredential());
    let token: TokenInfo = await new Promise(
      function (resolve, reject) {
        client.create({
          username: "admin",
          password: "admin",
          expiry: 60,
        }, (a) => {
          resolve(a);
        });
      },
    );
  });
}
