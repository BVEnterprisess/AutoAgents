#!/bin/bash

# Infrastructure Assassin - 15-Minute Sync Rule Compliance Checker
# Rule Master Mandatory: Code must be committed and pushed every 15 minutes

echo "🔄 INFRASTRUCTURE ASSASSIN - SYNC RULE COMPLIANCE CHECK"
echo "=========================================================="

# Get the timestamp of the last commit
LAST_COMMIT_TIME=$(git log -1 --format=%ct)

if [ -z "$LAST_COMMIT_TIME" ]; then
    echo "❌ No commits found - initialize repository first"
    exit 1
fi

# Get current time
CURRENT_TIME=$(date +%s)

# Calculate time difference in minutes
TIME_DIFF_MINUTES=$(( (CURRENT_TIME - LAST_COMMIT_TIME) / 60 ))

echo "📊 Last commit: ${TIME_DIFF_MINUTES} minutes ago"

# 15-minute RULE_MASTER compliance check
if [ "$TIME_DIFF_MINUTES" -gt 15 ]; then
    echo "⚠️  RULE_MASTER VIOLATION: ${TIME_DIFF_MINUTES} minutes since last commit"
    echo "🚨 15-minute sync rule breached - commit immediately!"
    echo ""
    echo "📋 Uncommitted changes:"
    git status --porcelain

    if [ -n "$(git status --porcelain)" ]; then
        echo ""
        echo "💾 Auto-committing all changes..."
        echo "Enter commit message:"
        read commit_message

        if [ -z "$commit_message" ]; then
            commit_message="INFRASTRUCTURE ASSASSIN - 15min sync compliance (auto-commit)"
        fi

        git add .
        git commit -m "$commit_message"

        echo "🚀 Pushing to private repository..."
        git push origin main

        echo "✅ RULE_MASTER COMPLIANCE RESTORED"
    else
        echo "ℹ️  No changes to commit - but sync rule still violated"
        echo "   Consider adding progress comments or documentation updates"
    fi
else
    echo "✅ RULE_MASTER COMPLIANT: Within 15-minute sync window"
    echo "   Next sync check required: $((15 - TIME_DIFF_MINUTES)) minutes"
fi

echo ""
echo "💰 Revenue Impact Reminder:"
echo "   Every minute delayed costs enterprise customers ~$69/hour saved"

# Show implementation progress
echo ""
echo "📈 Implementation Progress:"
if [ -f "implementation_plan.md" ]; then
    COMPLETED=$(grep -c "\[x\]" implementation_plan.md)
    TOTAL=$(grep -c "\[x\]\|\[ \]" implementation_plan.md)
    if [ "$TOTAL" -gt 0 ]; then
        PERCENTAGE=$((COMPLETED * 100 / TOTAL))
        echo "   Progress: ${PERCENTAGE}% (${COMPLETED}/${TOTAL} tasks)"
    fi
fi

echo ""
echo "🎯 Infrastructure Assassin continues... Forward momentum only."
