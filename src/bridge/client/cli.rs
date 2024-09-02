// TODO: Read credentials from .env and/or as command line argument
/// bitvm -d [depositor-secret-key] (depositor context)
/// bitvm -o [operator-secret-key] (operator context)
/// bitvm -v [verifier-secret-key] (verifier context)
/// bitvm -w [withdrawer-secret-key] (withdrawer context)

// TODO: Manual mode (interactive)
/// bitvm status
/// bitvm broadcast-peg-in-confirm 123 (graph id)

// TODO: Automatic mode
/// bitvm -a
/// It will automatically poll for status updates and sign or broadcast txns


/// ./bitvm -o
/// >
/// > status
/// > ...
/// > broadcast-peg-in-confirm 123


/// loop
///// wait for user input and enter
///// parse user input, run command, output result to terminal
///// repeat