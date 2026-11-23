import { assert } from "chai";

import { expectReverted, oneToken, toBN, tokenBalance } from "../helpers";
import { BookFixture, findHint, listOrders, setupBook } from "./fixture";
import { cancelOrder, placeOrder } from "./actions";

describe("05-orderbook-with-local-checks", () => {
  let env: BookFixture;

  beforeEach(async () => {
    env = await setupBook();
  });

  it("keeps the orders sorted from the highest price to the lowest", async () => {
    const [alice, bob, carol] = env.traders;
    for (const [trader, price] of [
      [alice, 10],
      [bob, 30],
      [carol, 20],
    ] as const) {
      const hint = findHint(await env.readBook(), toBN(price));
      await placeOrder(env, trader, toBN(price), oneToken(), hint);
    }

    const book = await env.readBook();
    const prices = listOrders(book).map((cell) =>
      book.cells[cell].price.toNumber(),
    );
    assert.deepEqual(prices, [30, 20, 10]);
    assert.equal(
      (await tokenBalance(env.context, env.vault)).toString(),
      oneToken().muln(3).toString(),
    );

    const bobsCell = listOrders(book)[0];
    await cancelOrder(env, bob, bobsCell);
    const after = await env.readBook();
    assert.deepEqual(
      listOrders(after).map((cell) => after.cells[cell].price.toNumber()),
      [20, 10],
    );
  });

  it("supports head, middle, and tail insertions, rejects wrong and stale hints, relinks on cancel, and rejects full book", async () => {
    const [alice, bob, carol] = env.traders;

    // 1. Insert first order (price 20) at head (hint = null)
    await placeOrder(env, alice, toBN(20), oneToken(), null);
    let book = await env.readBook();
    const cell20 = listOrders(book)[0];

    // 2. Insert higher price (price 30) at head (hint = null) -> [30, 20]
    await placeOrder(env, bob, toBN(30), oneToken(), null);
    book = await env.readBook();
    const cell30 = listOrders(book)[0];

    // 3. Insert lower price (price 10) at tail after cell20 -> [30, 20, 10]
    await placeOrder(env, carol, toBN(10), oneToken(), cell20);
    book = await env.readBook();
    assert.deepEqual(
      listOrders(book).map((c) => book.cells[c].price.toNumber()),
      [30, 20, 10],
    );

    // 4. Insert in middle: price 25 after cell30 (before cell20) -> [30, 25, 20, 10]
    await placeOrder(env, alice, toBN(25), oneToken(), cell30);
    book = await env.readBook();
    assert.deepEqual(
      listOrders(book).map((c) => book.cells[c].price.toNumber()),
      [30, 25, 20, 10],
    );

    // 5. Wrong hint refused: trying to put price 5 after cell30 (next is 25, which is > 5)
    await expectReverted(
      { revertedWith: { message: "BadHint" } },
      placeOrder(env, alice, toBN(5), oneToken(), cell30),
    );

    // 6. Stale hint refused: compute hint for price 22 (after cell25), then insert price 24 after cell25 first
    const staleHintFor22 = findHint(book, toBN(22));
    await placeOrder(env, bob, toBN(24), oneToken(), staleHintFor22);
    await expectReverted(
      { revertedWith: { message: "BadHint" } },
      placeOrder(env, carol, toBN(22), oneToken(), staleHintFor22),
    );

    // 7. Cancel middle order (bob's price 24) and verify relinking + reuse of freed cell
    const midCell = listOrders(await env.readBook())[2];
    const freedCellIdx = midCell;
    await cancelOrder(env, bob, midCell);
    const afterCancel = await env.readBook();
    assert.equal(afterCancel.cells[freedCellIdx].alive, 0);
    assert.equal(afterCancel.freeHead, freedCellIdx);

    const newHint = findHint(afterCancel, toBN(22));
    await placeOrder(env, carol, toBN(22), oneToken(), newHint);
    const afterReuse = await env.readBook();
    assert.equal(afterReuse.cells[freedCellIdx].alive, 1);

    // 8. Fill remaining cells up to capacity (64 cells) and verify BookFull is refused
    const currentCount = listOrders(await env.readBook()).length;
    for (let i = currentCount; i < 64; i++) {
      const snap = await env.readBook();
      const p = toBN(100 + i);
      const h = findHint(snap, p);
      await placeOrder(env, alice, p, oneToken(), h);
    }

    await expectReverted(
      { revertedWith: { message: "BookFull" } },
      placeOrder(env, alice, toBN(500), oneToken(), null),
    );
  });
});
