/* Intentionally empty. librdkafka 2.12's rdkafka_conf.c does
 * `#ifdef WITH_OAUTHBEARER_OIDC` while CMake defines it as 0, so it includes
 * <curl/curl.h> even when built without curl. Nothing from curl is used (every
 * other site checks `#if WITH_OAUTHBEARER_OIDC`), so an empty header lets
 * kitz build on systems without curl headers (Linux/musl). See .cargo/config.toml. */
