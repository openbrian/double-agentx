# AgentX JSON

Net-SNMP has a plugin architecture that facilitates adding OIDs to monitor to
the system level SNMP Agent.  There are a few ways to extend the agent, but one
of the easiest ways is to use the AgentX subagent.

Often written in C, this subagent is written in Rust.

AgentX Anything JSON is a Net-SNMP AgentX subagent that reads JSON data from any 
JSON source and maps elements in the JSON to OIDs.  This includes using data
types for the OID.

SNMP doesn't have support for floats.  One solution is to scale the float data
to an integer.  For example, 22.5 can be scaled to 22500.  This is done by 
multiplying the float by 1000.  The MIB also defines the scale factor as a
hint.  The value can be scaled back to the original value by dividing by the
scale factor. 

Currently, reads rocm-smi output, but the intent it to read any JSON based 
on a declarative config file.

# Build

```shell
cargo build
```


# Test

Could use some help with setting up unit tests.


# Run

Customize the config and run locally

```
cp template/config.yaml .
cargo run
```

Find the agentx socket file in /var, likely /var/agentx/master.


# Deploy

## Set up SNMPd

Add this to SNMPd's config /etc/snmp/snmpd.conf.

```
# Add your OID to the system view
view systemview included .1.3.6.1.4.1.<pen>

# enable agentx
master agentx

agentxsocket /tmp/agentx
agentxperms 770 770 <username> <groupname>
```

Load the MIB file.

TODO

Confirm the MIB file has the right structure.

```shell
snmptranslate -M.:/usr/share/snmp/mibs/ietf -Tp AMDGPU-MIB::doubleagentx
```

Run these commands.

```shell
cargo build --release
mkdir -p ~/.config/agentx-json/
cp target/release/agentx-json ~/.config/agentx-json/
cp config.yaml ~/.config/agentx-json/
```

Find the new binary in `target/release/agentx-json` and copy it to wherever you want.


# Systemd

After copying the service file and config file into place, customize them. 

```shell
cp agentx-json.service ~/.config/systemd/user/
cp config.yaml ~/.config/agentx-json/
systemctl daemon-reload
systemctl enable --user agentx-json.service
systemctl start --user agentx-json.service
```

Monitor logs

```shell
journalctl --user -fu agentx
```

# Acknowledgements

Thanks to LINBIT for creating the [agentx rust crate](https://crates.io/crates/agentx) and sharing a 
[demonstration of it](https://github.com/LINBIT/drbd-reactor/blob/master/src/plugin/agentx.rs) 
in [drbd-reactor](https://github.com/LINBIT/drbd-reactor/blob/master/src/plugin/agentx.rs).
