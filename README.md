# t3rn subxt 

That's a fork of original [substrate-subxt](https://github.com/paritytech/substrate) - a library to **sub**mit e**xt**rinsics to a [substrate](https://github.com/paritytech/substrate) node via RPC.

It extends the original with ways to interact with [gateways](../../gateway) i.e.:
- create & sign transaction dispatchable for `multistep_call`
- send calls for `multistep_call`
- watch for events of after-execution of `multistep_call`

#### License

<sup>
The entire code within this repository is licensed under the <a href="LICENSE">GPLv3</a>.
Please <a href="https://www.parity.io/contact/">contact us</a> if you have questions about the licensing of our
 products.
</sup>
