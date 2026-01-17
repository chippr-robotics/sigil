# Recovery Procedures

This document outlines recovery procedures for various failure scenarios in the Sigil system.

## 1. Scenario Categories

| Category | Severity | Examples |
|----------|----------|----------|
| CRITICAL | System unusable | Master shard loss, mother compromise |
| HIGH | Bounded loss possible | Disk theft, agent compromise |
| MEDIUM | Operational impact | Disk corruption, presig exhaustion |
| LOW | Minor inconvenience | Disk read errors, daemon restart |

## 2. Recovery Procedures

### 2.1 CRITICAL: Master Cold Shard Loss

**Symptoms:**
- Mother device destroyed or inaccessible
- Master cold shard backup unavailable

**Impact:**
- Cannot create new children
- Cannot refill existing children
- Existing presigs remain usable until exhausted

**Recovery:**
1. **Immediate**: Assess remaining presig inventory across all children
2. **Short-term**: Plan transition to new master
3. **Long-term**: Generate new master, create new children, migrate funds

**Prevention:**
- Store encrypted master shard backups in multiple secure locations
- Use Shamir's Secret Sharing for backup distribution
- Regular backup verification

---

### 2.2 CRITICAL: Master Agent Shard Loss

**Symptoms:**
- Agent server catastrophic failure
- Backup unavailable or corrupted

**Impact:**
- Cannot sign with any child (even with disks present)
- All presigs become unusable

**Recovery:**
1. **Immediate**: Restore from backup if available
2. **If no backup**: System is effectively bricked
3. **Funds recovery**: If funds are in smart contracts with timelocks, wait for recovery paths

**Prevention:**
- Encrypted backups in multiple locations
- Consider HSM storage for agent shard
- Regular backup testing

---

### 2.3 CRITICAL: Mother Device Compromise

**Symptoms:**
- Unauthorized access detected
- Unexpected children in registry
- Anomalous presigs detected

**Impact:**
- All future children potentially compromised
- Existing children may have backdoored presigs

**Recovery:**
1. **Immediate**: Physically secure/destroy the mother device
2. **Assessment**: Audit all children created since potential compromise
3. **Nullification**: Nullify all suspicious children
4. **Migration**: Create new master on new air-gapped device
5. **Funds**: Move funds from compromised children to new ones

**Prevention:**
- Physical security controls
- Multi-person ceremonies
- Tamper-evident seals

---

### 2.4 HIGH: Agent Daemon Compromise

**Symptoms:**
- Unauthorized signing detected
- Anomalous RPC activity
- Unexpected process behavior

**Impact:**
- Attacker can sign when disk is present
- Agent shares exposed

**Recovery:**
1. **Immediate**: Stop the daemon
2. **Isolation**: Disconnect from network
3. **Assessment**: Review signing logs
4. **Wipe**: Securely delete agent store
5. **Rebuild**: Fresh installation on clean system
6. **Reload**: Transfer new agent shares from mother

**Prevention:**
- Minimal attack surface
- Regular security updates
- Process isolation/sandboxing
- Monitoring and alerting

---

### 2.5 HIGH: Floppy Disk Stolen

**Symptoms:**
- Disk missing
- Reconciliation shows unknown usage (if recovered)

**Impact:**
- Attacker has up to N presig cold shares
- Cannot sign without agent shares (secure)

**Recovery:**
1. **Immediate**: Report to security team
2. **Nullification**: Nullify the child via mother
3. **Agent cleanup**: Delete agent shares for that child
4. **New child**: Create replacement child if needed
5. **Monitoring**: Watch for unauthorized transaction attempts

**Prevention:**
- Physical security awareness training
- Tamper-evident storage
- Lower presig count for high-risk environments

---

### 2.6 HIGH: Disk Cloning Detected

**Symptoms:**
- Reconciliation shows divergent usage logs
- Presig count mismatch
- Same presig index used twice

**Impact:**
- Attacker attempted to use cloned disk
- Private key potentially exposed if same R used twice

**Recovery:**
1. **Immediate**: Nullify the child
2. **Analysis**: Check if any presigs were reused with different messages
3. **If key exposed**: Immediately move all funds
4. **Investigation**: Determine how cloning occurred
5. **New child**: Create replacement with enhanced procedures

**Prevention:**
- Agent-side presig tracking
- Frequent reconciliation
- Tamper-evident disk storage

---

### 2.7 MEDIUM: Disk Corruption

**Symptoms:**
- Disk read errors
- Header validation failure
- Presig table corruption

**Impact:**
- Disk unusable
- Presigs lost (bounded loss)

**Recovery:**
1. **Assessment**: Determine extent of corruption
2. **Partial recovery**: If header intact, may recover some presigs
3. **Agent state**: Agent shares still valid
4. **Refill**: Return to mother for new presigs (if disk salvageable)
5. **Replace**: Create new child if disk unsalvageable

**Prevention:**
- Use quality media
- Periodic disk health checks
- Don't exceed insertion cycle limits
- Keep backup presig batches at mother

---

### 2.8 MEDIUM: Presig Exhaustion

**Symptoms:**
- "No presigs available" error
- Presig count reaches zero

**Impact:**
- Cannot sign until refilled
- Operational disruption

**Recovery:**
1. **Planned**: Return disk to mother for refill ceremony
2. **Emergency**: If mother unavailable, use emergency reserve
3. **Handoff**: Transfer signing responsibility to another child

**Prevention:**
- Monitor presig counts
- Set warning thresholds (e.g., 100 remaining)
- Schedule regular refill ceremonies
- Maintain multiple children for redundancy

---

### 2.9 MEDIUM: Reconciliation Deadline Passed

**Symptoms:**
- Validation error: "Reconciliation deadline passed"
- Cannot sign despite presigs remaining

**Impact:**
- Disk locked until reconciliation
- Signing blocked

**Recovery:**
1. **Return to mother**: Perform reconciliation ceremony
2. **Audit**: Review all activity since last reconciliation
3. **Clear**: Reset deadline after successful reconciliation
4. **Refill**: Optionally add new presigs

**Prevention:**
- Calendar reminders for reconciliation deadlines
- Automated monitoring of deadline proximity
- Set reasonable deadlines (not too short)

---

### 2.10 LOW: Daemon Restart Required

**Symptoms:**
- IPC connection refused
- Signing requests timeout

**Impact:**
- Temporary signing unavailability

**Recovery:**
1. **Check status**: `systemctl status sigil-daemon`
2. **Review logs**: `journalctl -u sigil-daemon`
3. **Restart**: `systemctl restart sigil-daemon`
4. **Verify**: Test with `sigil status`

**Prevention:**
- Process monitoring
- Automatic restart on failure
- Health check endpoints

---

### 2.11 LOW: Disk Read Errors (Transient)

**Symptoms:**
- Occasional read failures
- Retry succeeds

**Impact:**
- Slower operations
- Potential data integrity risk

**Recovery:**
1. **Retry**: Most transient errors resolve
2. **Clean**: Clean disk contacts
3. **Test**: Run full disk validation
4. **Replace**: If errors persist, plan disk replacement

**Prevention:**
- Handle disks carefully
- Use quality USB floppy drives
- Regular disk health monitoring

---

## 3. Emergency Contacts

| Role | Contact | Responsibility |
|------|---------|----------------|
| Security Lead | [TBD] | Compromise response |
| Operations | [TBD] | Disk/daemon issues |
| Mother Custodian | [TBD] | Refill ceremonies |

## 4. Recovery Drill Schedule

| Drill Type | Frequency | Last Performed |
|------------|-----------|----------------|
| Backup restoration | Quarterly | - |
| Disk loss simulation | Bi-annually | - |
| Full disaster recovery | Annually | - |

## 5. Audit Trail

All recovery actions should be logged:

```
Date: YYYY-MM-DD
Incident: [Description]
Severity: [CRITICAL/HIGH/MEDIUM/LOW]
Actions Taken:
  1. [Action]
  2. [Action]
Outcome: [Result]
Follow-up: [Required actions]
Performed By: [Name]
Witnessed By: [Name] (for CRITICAL/HIGH)
```

## 6. Changelog

- **v0.1.0** (2026-01-17): Initial recovery procedures
