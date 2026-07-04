# Troubleshooting Guide

This guide helps diagnose and resolve common issues with `http-traffic-sim`.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Configuration Issues](#configuration-issues)
- [Connection Issues](#connection-issues)
- [Performance Issues](#performance-issues)
- [Stress Testing Issues](#stress-testing-issues)
- [Discovery Issues](#discovery-issues)
- [Error Analysis](#error-analysis)
- [Platform-Specific Issues](#platform-specific-issues)

---

## Installation Issues

### Cargo Build Fails

**Symptom**: `cargo build` fails with compilation errors

**Solutions**:

1. **Update Rust toolchain**:
   ```bash
   rustup update stable
   cargo --version  # Should be 1.70+
   ```

2. **Clean build cache**:
   ```bash
   cargo clean
   cargo build --release
   ```

3. **Check dependencies**:
   ```bash
   cargo tree | grep -i "duplicate\|conflict"
   ```

### OpenSSL Linking Errors (Linux)

**Symptom**: `error: linking with cc failed` mentioning OpenSSL

**Solution**:
```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora/RHEL
sudo dnf install openssl-devel

# Arch
sudo pacman -S openssl pkg-config
```

### macOS Build Issues

**Symptom**: Certificate verification errors during build

**Solution**:
```bash
# Update Xcode command line tools
xcode-select --install

# Update certificates
/Applications/Python\ 3.*/Install\ Certificates.command
```

---

## Configuration Issues

### "Target URL is required"

**Symptom**: Error on startup even with `--url` flag

**Cause**: Configuration validation failing

**Solutions**:

1. **Check URL format**:
   ```bash
   # ✅ Correct
   --url https://example.com/api

   # ❌ Wrong
   --url example.com  # Missing scheme
   ```

2. **Check config file conflicts**:
   ```bash
   # Remove config file temporarily
   mv config.yaml config.yaml.bak

   # Try CLI only
   cargo run -- --url https://example.com --concurrent 10 --duration 30
   ```

### "Failed to parse YAML config"

**Symptom**: YAML parsing errors

**Common Issues**:

1. **Indentation errors**:
   ```yaml
   # ❌ Wrong - inconsistent indentation
   target:
     url: "https://example.com"
      method: "GET"

   # ✅ Correct - consistent 2-space indentation
   target:
     url: "https://example.com"
     method: "GET"
   ```

2. **Quotes in strings**:
   ```yaml
   # ❌ Wrong - unescaped quotes
   body: "{"test": "value"}"

   # ✅ Correct - escaped or single quotes
   body: '{"test": "value"}'
   ```

3. **Type mismatches**:
   ```yaml
   # ❌ Wrong - string instead of number
   concurrent: "50"

   # ✅ Correct - number without quotes
   concurrent: 50
   ```

**Debugging**:
```bash
# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('config.yaml'))"
```

### Stress testing errors

Since authorization has been removed, stress tests will run if a valid `stress_pattern` is provided (assuming you are authorized). Errors are typically due to invalid config, missing `target.url`, or safety limits being exceeded.

See the "Stress Testing" section in [docs/DOCUMENTATION.md](docs/DOCUMENTATION.md) for current requirements.

---

## Connection Issues

### "Connection refused (ECONNREFUSED)"

**Symptom**: All requests fail with connection refused

**Causes & Solutions**:

1. **Target is down**:
   ```bash
   # Test manually
   curl https://example.com
   ```

2. **Firewall blocking**:
   ```bash
   # Check if port is accessible
   telnet example.com 443
   nc -zv example.com 443
   ```

3. **Wrong port**:
   ```yaml
   # Check port in URL
   url: "https://example.com:443"  # HTTPS = 443
   url: "http://example.com:80"    # HTTP = 80
   ```

4. **Target blocking traffic**:
   - Check if target has rate limiting
   - Reduce concurrent requests
   - Add delays between requests

### "Connection timeout (ETIMEDOUT)"

**Symptom**: Requests timing out

**Solutions**:

1. **Increase timeout**:
   ```bash
   --timeout 60  # Increase to 60 seconds
   ```

   Or in config:
   ```yaml
   client:
     timeout_secs: 60
   ```

2. **Check network latency**:
   ```bash
   ping example.com
   traceroute example.com
   ```

3. **Reduce concurrent load**:
   ```yaml
   pattern:
     type: fixed
     concurrent: 10  # Start low, increase gradually
   ```

### "TLS handshake failed"

**Symptom**: HTTPS requests failing with TLS errors

**Solutions**:

1. **Update certificates**:
   ```bash
   # macOS
   brew install ca-certificates

   # Linux
   sudo update-ca-certificates
   ```

2. **Test certificate manually**:
   ```bash
   openssl s_client -connect example.com:443 -servername example.com
   ```

3. **Check TLS version**:
   ```bash
   # Some old servers only support TLS 1.0/1.1
   # Modern clients require TLS 1.2+
   nmap --script ssl-enum-ciphers -p 443 example.com
   ```

### "Too many open files"

**Symptom**: Failures with high concurrency

**Cause**: OS file descriptor limit

**Solutions**:

1. **Check current limit**:
   ```bash
   ulimit -n
   ```

2. **Increase limit temporarily**:
   ```bash
   # Linux/macOS
   ulimit -n 10000
   ```

3. **Increase permanently** (Linux):
   ```bash
   # Edit /etc/security/limits.conf
   * soft nofile 10000
   * hard nofile 10000
   ```

4. **Increase permanently** (macOS):
   ```bash
   # Edit /Library/LaunchDaemons/limit.maxfiles.plist
   sudo launchctl limit maxfiles 10000 10000
   ```

---

## Performance Issues

### Low Throughput (RPS)

**Symptom**: Requests per second lower than expected

**Causes & Solutions**:

1. **Connection pooling disabled**:
   ```yaml
   client:
     pool_max_idle_per_host: 128  # Enable pooling
   ```

2. **High latency to target**:
   ```bash
   # Measure latency
   ping -c 10 example.com

   # If high, reduce concurrent requests
   concurrent: 10  # Lower concurrency for high-latency targets
   ```

3. **Target rate limiting**:
   - Monitor for 429 (Too Many Requests) responses
   - Use rate-limited pattern instead of fixed:
   ```yaml
   pattern:
     type: ratelimit
     rate: 100  # Max 100 req/s
   ```

4. **CPU bottleneck**:
   ```bash
   # Monitor CPU during test
   top
   htop

   # If maxed out, reduce concurrency or distribute load
   ```

### High Memory Usage

**Symptom**: Memory usage growing during test

**Causes & Solutions**:

1. **Large response bodies**:
   - Target returns large payloads
   - Responses stored in memory for metrics

   **Solution**: Not currently configurable, but future improvement

2. **Very long tests**:
   - Latencies accumulated in memory

   **Solution**: Use shorter test durations or periodic resets

3. **Memory leak** (unlikely):
   ```bash
   # Monitor memory over time
   watch -n 1 'ps aux | grep http-traffic-sim'

   # If growing without bound, file a bug report
   ```

### Inconsistent Results

**Symptom**: Different metrics on repeated tests

**Causes**:

1. **Network variability**: Normal
2. **Target load variability**: Target busy at different times
3. **Local system load**: Other processes competing

**Solutions**:

1. **Run multiple iterations**:
   ```bash
   for i in {1..5}; do
     cargo run --release -- --url https://example.com --duration 60 --output results-$i.json
   done
   ```

2. **Use larger sample sizes**:
   ```yaml
   pattern:
     type: fixed
     concurrent: 50
     duration_secs: 300  # Longer test = more stable results
   ```

3. **Control for external factors**:
   - Run at consistent times
   - Close other applications
   - Use dedicated test environment

---

## Stress Testing Issues

### Stress test configuration issues

**Symptom**: Errors when starting a stress test

**Solution**: Ensure `stress_pattern` is correctly specified and (if used) `safety_limits` are not exceeded. See the Stress Testing section in DOCUMENTATION.md.

### "Connection rate exceeds safety limit"

**Symptom**: Validation error on startup

**Solution**: Adjust safety limits:
```yaml
stress_pattern:
  category: connectionflood
  connections_per_second: 1000  # Your desired rate

safety_limits:
  max_connections_per_second: 1000  # Must be >= pattern rate
```

### Target Appears Unaffected by Stress Test

**Symptom**: Stress test runs but target responds normally

**Possible Causes**:

1. **Target has good protection**:
   - Rate limiting working
   - Load balancer distributing load
   - Connection limits enforced
   - **This is good! Your target is resilient**

2. **Test too weak**:
   ```yaml
   # Increase intensity gradually
   stress_pattern:
     category: connectionflood
     connections_per_second: 500  # Increase from 100
     hold_time_ms: 10000           # Increase from 5000
   ```

3. **Target in cloud** (CDN/WAF):
   - May have DDoS protection
   - May be blocking your IP
   - Check target logs for blocks

**Important**: Only test systems you own or have authorization to test!

---

## Discovery Issues

### "No addresses resolved"

**Symptom**: DNS resolution failing during discovery

**Solutions**:

1. **Check DNS**:
   ```bash
   nslookup example.com
   dig example.com
   ```

2. **Try IP address**:
   ```yaml
   target:
     url: "http://192.168.1.100:8080"  # Use IP directly
   ```

3. **Check /etc/hosts**:
   ```bash
   grep example.com /etc/hosts
   ```

### Discovery Takes Too Long

**Symptom**: Discovery phase hangs or is very slow

**Causes & Solutions**:

1. **Large port range**:
   ```yaml
   discovery:
     ports:
       start: 8000
       end: 9000  # 1000 ports = slow
   ```

   **Solution**: Narrow the range or use list:
   ```yaml
   discovery:
     ports: [80, 443, 8080, 8443]  # Only check specific ports
   ```

2. **Timeouts not reached**:
   ```yaml
   discovery:
     timeout_ms: 2000  # Reduce if confident about network
     retries: 1        # Reduce retries
   ```

3. **Network filtering**:
   - Firewall dropping packets (no response)
   - Leads to full timeout waits

   **Solution**: Use shorter timeouts or skip filtered ports

### All Ports Showing as Filtered

**Symptom**: Discovery reports all ports as filtered/unreachable

**Causes**:

1. **Firewall blocking scans**:
   - Common on cloud providers
   - Security groups may block

2. **Target behind NAT/load balancer**:
   - Only specific ports exposed

3. **Rate limiting**:
   - Too many connection attempts

**Solutions**:

1. **Validate single port first**:
   ```yaml
   discovery:
     mode: validate
     ports: 443  # Just one port
   ```

2. **Use on_failure: warn**:
   ```yaml
   discovery:
     on_failure: warn  # Continue despite failures
   ```

3. **Disable discovery**:
   ```yaml
   discovery:
     enabled: false  # Skip if not needed
   ```

---

## Error Analysis

### Understanding Error Distribution

**High Error Rates (>5%)**:

1. **Check error types**:
   ```
   ERROR DISTRIBUTION
   --------------------------------------------------
   Connection refused: 450 (45.0%)
   Timeout: 350 (35.0%)
   ```

2. **Connection refused** = Target down or port closed
3. **Timeout** = Target slow or network issues
4. **Connection reset** = Target or firewall killing connections
5. **TLS errors** = Certificate or protocol issues

### Debugging Specific Errors

**400 Bad Request**:
- Check headers and body format
- Validate Content-Type header
- Check for required headers

**401 Unauthorized**:
- Add authentication headers
- Check API key/token validity

**403 Forbidden**:
- Check IP allowlist
- Rate limit may be blocking
- Check request origin

**404 Not Found**:
- Verify URL path
- Check for typos in endpoint

**429 Too Many Requests**:
- Target rate limiting enforced
- Reduce request rate
- Add delays between requests

**500 Internal Server Error**:
- Target application error
- Check target logs
- May indicate target under stress

**502 Bad Gateway / 503 Service Unavailable**:
- Load balancer or proxy issue
- Target overloaded
- Reduce load intensity

**504 Gateway Timeout**:
- Upstream service slow
- Increase timeout
- Reduce concurrent load

---

## Platform-Specific Issues

### Linux

**Issue**: Permission denied on low ports (<1024)
```bash
# Solution: Use higher port or run with sudo
--url http://localhost:8080  # Use port > 1024
```

**Issue**: `EMFILE` (too many open files)
```bash
# Solution: Increase file descriptor limit
ulimit -n 10000
```

### macOS

**Issue**: Gatekeeper blocking binary
```bash
# Solution: Allow in Security & Privacy settings
xattr -d com.apple.quarantine /path/to/binary
```

**Issue**: Network performance degradation
```bash
# Solution: Adjust network buffers
sudo sysctl -w net.inet.tcp.sendspace=65536
sudo sysctl -w net.inet.tcp.recvspace=65536
```

### Windows

**Issue**: Long file paths
```bash
# Solution: Enable long path support
# Run as administrator:
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" -Name "LongPathsEnabled" -Value 1
```

**Issue**: Firewall blocking outbound connections
```bash
# Solution: Add firewall exception
# Windows Defender Firewall > Allow an app
```

---

## Getting Help

### Before Reporting Issues

1. **Collect information**:
   ```bash
   # System info
   uname -a
   cargo --version
   rustc --version

   # Run with debug output
   RUST_LOG=debug cargo run -- [your args] 2>&1 | tee debug.log
   ```

2. **Create minimal reproduction**:
   - Simplest config that reproduces issue
   - Remove unrelated configuration
   - Test with public endpoint (if possible)

3. **Check existing issues**:
   - Search GitHub issues for similar problems
   - Check closed issues for solutions

### Reporting Bugs

Include:
- Operating system and version
- Rust and cargo versions
- Full command or config used
- Complete error messages
- Steps to reproduce

### Community Support

- **GitHub Issues**: https://github.com/anthropics/http-traffic-sim/issues
- **Documentation**: README.md, ARCHITECTURE.md
- **Examples**: config.*.example.yaml files

---

## Quick Reference

### Common Commands

```bash
# Test with minimal settings
cargo run --release -- --url https://example.com --concurrent 10 --duration 30

# Test with config file
cargo run --release -- --config config.yaml

# Enable debug logging
RUST_LOG=debug cargo run --release -- --config config.yaml

# Run tests
cargo test

# Run benchmarks
cargo bench

# Build optimized binary
cargo build --release
```

### Health Check Workflow

1. **Verify target is reachable**:
   ```bash
   curl -i https://example.com
   ```

2. **Test with minimal load**:
   ```bash
   cargo run --release -- --url https://example.com --concurrent 1 --duration 10
   ```

3. **Gradually increase load**:
   ```bash
   --concurrent 10
   --concurrent 50
   --concurrent 100
   ```

4. **Monitor target**:
   - Watch CPU, memory, response times
   - Check for errors in target logs
   - Verify target health endpoints

5. **Analyze results**:
   - Check success rate (should be >95%)
   - Review latency percentiles
   - Look for error patterns

### Performance Tuning Checklist

- [ ] Connection pooling enabled
- [ ] Appropriate timeout configured
- [ ] Concurrency matches target capacity
- [ ] Rate limiting if target enforces limits
- [ ] File descriptor limits increased (if high concurrency)
- [ ] Running release build (not debug)
- [ ] Network latency measured
- [ ] Target has capacity for load

---

**Last Updated**: March 17, 2026
**For Updates**: Check GitHub repository
