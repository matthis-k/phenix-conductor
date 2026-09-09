_: {
  perSystem =
    { config, pkgs, ... }:
    let
      luaBinding = config.packages.phenix-binding-lua;
      observableCallbackFixture = pkgs.rustPlatform.buildRustPackage {
        pname = "phenix-observable-callback-fixture";
        version = "0";
        src = pkgs.lib.cleanSource ../rust;
        cargoLock.lockFile = ../rust/Cargo.lock;
        cargoBuildFlags = [
          "--package"
          "phenix-acp-stdio"
          "--example"
          "observable_callback_fixture"
        ];
        doCheck = false;
        installPhase = ''
          runHook preInstall
          executable="$(find target -path '*/release/examples/observable_callback_fixture' -type f -print -quit)"
          test -n "$executable"
          mkdir -p "$out/bin"
          cp "$executable" "$out/bin/observable_callback_fixture"
          runHook postInstall
        '';
      };
    in
    {
      checks.phenix-binding-lua-observable-callback =
        pkgs.runCommand "phenix-binding-lua-observable-callback-check"
          {
            nativeBuildInputs = [
              pkgs.luajit
              luaBinding
              observableCallbackFixture
            ];
          }
          ''
            export LUA_CPATH="${luaBinding}/lib/lua/5.1/?.so;;"
            luajit - <<'LUA'
            local phenix = require("phenix")
            local client = phenix.connect({
              command = "${observableCallbackFixture}/bin/observable_callback_fixture",
            })

            local function pack(...)
              return { n = select("#", ...), ... }
            end

            local function fail(label, err)
              if type(err) == "table" and err.message ~= nil then
                error(label .. ": " .. err.message)
              end
              error(label .. ": " .. tostring(err))
            end

            local function poll_client()
              local ok, err = pcall(function()
                client:poll()
              end)
              if not ok then
                fail("client poll", err)
              end
            end

            local function await(request, label)
              for _ = 1, 1000000 do
                poll_client()
                local result = pack(request:poll())
                if result.n > 0 then
                  if result[2] ~= nil then
                    fail(label, result[2])
                  end
                  return result[1]
                end
              end
              error(label .. ": timed out")
            end

            local negotiated = false
            for _ = 1, 1000000 do
              if client:extensions()["phenix.application.sdk-get@1"] then
                negotiated = true
                break
              end
              poll_client()
            end
            assert(negotiated, "SDK extension was not negotiated")

            local sdk = await(client:sdk(), "sdk get")
            assert(type(sdk) == "table")
            assert(type(sdk.fixture) == "table")
            assert(type(sdk.fixture.state) == "table")
            assert(type(sdk.fixture.state.get) == "function")
            assert(type(sdk.fixture.state.listen) == "function")

            local current = await(sdk.fixture.state.get(), "observable get")
            assert(current.value == 41)
            assert(current.version == 0)

            local calls = 0
            local listen = sdk.fixture.state.listen({
              initial = { kind = "full" },
              mode = { kind = "full" },
              path = { segments = {} },
              scope = { kind = "recursive" },
            }, function(delivery)
              calls = calls + 1
              assert(type(delivery) == "userdata")
              assert(tostring(delivery) == "<phenix observable delivery>")
              assert(delivery:commit_id() == nil)
              assert(delivery:value_id() == "fixture.state@1")
              assert(delivery:from_version() == 0)
              assert(delivery:version() == 0)
              assert(delivery:subscription_id() == 1)
              assert(delivery:generation() == 1)

              local payload = delivery:payload()
              assert(payload.kind == "Full")
              assert(payload.value == 41)
            end)

            local pending = pack(listen:poll())
            assert(pending.n == 0, "listen completed before the host serviced its callback")
            assert(calls == 0, "Lua callback ran outside the host polling point")

            local stop = await(listen, "observable listen")
            assert(calls == 1)
            assert(type(stop) == "function")
            await(stop(), "observable stop")
            LUA
            touch "$out"
          '';
    };
}
