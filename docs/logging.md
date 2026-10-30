```bash
# Show backend Pods
sudo k3s kubectl get pods -l app=yalom-bot

# Recent logs for the running backend
sudo k3s kubectl logs deployment/yalom-bot -c yalom-bot --since=30m --timestamps --tail=500

# Follow logs while reproducing an issue
sudo k3s kubectl logs -f deployment/yalom-bot -c yalom-bot --timestamps

# Logs for a specific Pod, including its previous container if it restarted
sudo k3s kubectl logs POD_NAME -c yalom-bot --timestamps --tail=500
sudo k3s kubectl logs POD_NAME -c yalom-bot --previous --timestamps --tail=500
```
