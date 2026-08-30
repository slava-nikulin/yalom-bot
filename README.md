# yalom-bot

Ambient Telegram bot powered by LLM. Subtle existential nudges against everyday automatism

# TODO

новая VM:

1. Yalom.Role=app-node → Dynamic Group → Vault
2. добавить VNIC в NSG yalom-k3s-nodes → NLB имеет право ходить на неё
3. удалить старый backend из NLB и добавить новый private IP
4. запустить k3s bootstrap: `./cluster/k3s/bootstrap/bootstrap.sh oracle-frankfurt`
5. обновить переменные на gh: `./scripts/configure-gh-cd.sh oracle-frankfurt`
