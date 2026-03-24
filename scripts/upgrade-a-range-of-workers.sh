for i in 30 31 32 33 34 35; do
    echo ""
    echo "=========================================================="
    echo " Starting upgrade: worker-$i"
    echo "=========================================================="
    ansible-playbook ansible/playbooks/operations/upgrade-worker.yml -e target_host=worker-$i -v || { echo "FAILED on worker-$i — stopping"; exit 1; }
    echo "=========================================================="
    echo " Completed upgrade: worker-$i"
    echo "=========================================================="
done
echo ""
echo "All done."