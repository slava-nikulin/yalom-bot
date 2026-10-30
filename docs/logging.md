```bash
sudo k3s kubectl get pods -l app=yalom-bot
sudo k3s kubectl logs deployment/yalom-bot -c yalom-bot --since=30m --timestamps --tail=500
sudo k3s kubectl logs -f deployment/yalom-bot -c yalom-bot --timestamps
sudo k3s kubectl logs POD_NAME -c yalom-bot --timestamps --tail=500
sudo k3s kubectl logs POD_NAME -c yalom-bot --previous --timestamps --tail=500
```
